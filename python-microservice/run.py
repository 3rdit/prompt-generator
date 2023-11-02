import uvicorn
from app import app

import gmail_setup
import sentiment_setup
import email_processing

'''
TODO:
* Add a way to pass a GmailAPI object to the email_processing.py file so that it can be used to send emails for specific gmail accounts
* Implement the AI logic from rust into python (if we are fully commited to using python)
* Add sentiment checking to emails properly
'''

@app.get("/")
def read_root():
    return "root"

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8000)