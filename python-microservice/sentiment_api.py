import joblib
from sklearn.feature_extraction.text import CountVectorizer
from bs4 import BeautifulSoup
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
import pandas as pd
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import train_test_split


def preprocess_review(review):
    soup = BeautifulSoup(review, "html.parser")
    review = soup.get_text()
    review = review.lower()
    review = ''.join(char for char in review if char.isalpha() or char.isspace())
    return review


class SentimentAPI:
    def __init__(self):
        self.is_initialised = False
        try:
            self.clf = joblib.load("bin\\model.pkl")
            self.vectorizer = joblib.load("bin\\vectorizer.pkl")
            self.is_initialised = True
        except:
            self.clf = None
            self.vectorizer = None

    def predict(self, statement: str):
        if not self.clf or not self.vectorizer:
            raise HTTPException(status_code=500, detail="Model not loaded!")

        processed_statement = preprocess_review(statement)
        statement_vector = self.vectorizer.transform([processed_statement])
        prediction = self.clf.predict(statement_vector)

        return str(prediction[0])

    def train(self):
        if self.is_initialised:
            return None

        data = pd.read_csv('bin\\IMDB_Dataset.csv')

        reviews = data['review'].values
        sentiments = data['sentiment'].values

        reviews = [preprocess_review(review) for review in reviews]

        X_train, X_test, y_train, y_test = train_test_split(reviews, sentiments, test_size=0.2, random_state=42)
        self.vectorizer = CountVectorizer(stop_words='english', max_features=10000)
        X_train_vec = self.vectorizer.fit_transform(X_train)

        self.clf = LogisticRegression(solver='liblinear', max_iter=1000)
        self.clf.fit(X_train_vec, y_train)

        # Save the trained model and vectorizer for future use
        joblib.dump(self.clf, "bin\\model.pkl")
        joblib.dump(self.vectorizer, "bin\\vectorizer.pkl")