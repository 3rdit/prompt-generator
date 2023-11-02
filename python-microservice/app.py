from fastapi import FastAPI
from gmail_api import GmailAPI

app = FastAPI()

gmail = GmailAPI()