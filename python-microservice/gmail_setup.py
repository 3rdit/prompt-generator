from fastapi import FastAPI, Depends
from pydantic import BaseModel
from sentiment_api import SentimentAPI
from typing import List
from app import app, gmail

class GetLatestMail(BaseModel):
    amount: int
        
class UserID(BaseModel):
    userid: str

class ContentMail(BaseModel):
    ids: List[str]

class SendMail(BaseModel):
    uid: str
    recipient: str
    subject: str
    body: str
    
class ReplyMail(BaseModel):
    uid: str
    message_id: str
    body: str

@app.post("/gmail/setup/")
async def setup_gmail(uid: UserID):
    gmail.get_service(uid.userid)
    return {"status": f"Gmail account has been set up, and saved for user {uid.userid}"}

@app.post("/gmail/get_mail")
async def get_mail(v: GetLatestMail):
    return {"emails:": gmail.list_emails(v.amount)}

@app.post("/gmail/get_mail/content")
def get_real_mail(message_ids: ContentMail):
    return {"emails": gmail.list_emails_with_details(message_ids.ids)}

@app.post("/gmail/send_mail")
def send_mail(to_email: str, subject: str, body: str):
    gmail.send_email(to_email, subject, body)
    return {"status": "Email sent!"}    