from fastapi import FastAPI, Depends
from pydantic import BaseModel
from sentiment_api import SentimentAPI
from app import app

sentiment = SentimentAPI()

class Statement(BaseModel):
    text: str

@app.post("/sent/predict/")
async def predict(statement: Statement):
    return {"prediction": sentiment.predict(statement.text)}

@app.get("/sent/train/")
async def train():
    if sentiment.train() == None:
        return {"status": "Already trained!"}
    return {"status": "Training completed."}