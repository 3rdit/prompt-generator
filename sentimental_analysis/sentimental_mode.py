from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
import joblib
from sklearn.feature_extraction.text import CountVectorizer
from sklearn.linear_model import LogisticRegression
import pandas as pd
from sklearn.model_selection import train_test_split
from bs4 import BeautifulSoup

app = FastAPI()

class Statement(BaseModel):
    text: str

def preprocess_review(review):
    soup = BeautifulSoup(review, "html.parser")

    review = soup.get_text()
    review = review.lower()
    review = ''.join(char for char in review if char.isalpha() or char.isspace())
    return review

try:
    clf = joblib.load("model.pkl")
    vectorizer = joblib.load("vectorizer.pkl")
except:
    clf = None
    vectorizer = None


@app.get("/")
def read_root():
    return {"message": "i exist"}

@app.post("/predict/")
async def predict(statement: Statement):
    if not clf or not vectorizer:
        raise HTTPException(status_code=500, detail="Model not loaded!")
    
    processed_statement = preprocess_review(statement.text)
    statement_vector = vectorizer.transform([processed_statement])
    prediction = clf.predict(statement_vector)
    
    return {"prediction": str(prediction[0])}

@app.post("/train/")
def train():
    # TODO: ensure this method can't be called by just any user, as it is resource-intensive.
    
    data = pd.read_csv('IMDB_Dataset.csv')
    reviews = data['review'].values
    sentiments = data['sentiment'].values
    reviews = [preprocess_review(review) for review in reviews]
    
    X_train, X_test, y_train, y_test = train_test_split(reviews, sentiments, test_size=0.2, random_state=42)
    global vectorizer
    vectorizer = CountVectorizer(stop_words='english', max_features=10000)
    X_train_vec = vectorizer.fit_transform(X_train)
    
    global clf
    clf = LogisticRegression(solver='liblinear', max_iter=1000)
    clf.fit(X_train_vec, y_train)
    
    # Save the trained model and vectorizer for future use
    joblib.dump(clf, "model.pkl")
    joblib.dump(vectorizer, "vectorizer.pkl")
    
    return {"status": "Training completed."}