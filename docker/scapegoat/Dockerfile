FROM python:latest

COPY /src .

COPY ./requirements.txt .

EXPOSE 9000

RUN pip install -r requirements.txt
CMD python main.py

