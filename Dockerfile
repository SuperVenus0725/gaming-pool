# pull official base image
FROM python:3.8.4-alpine
RUN echo "Test Engine Ver 1.0"
# set work directory
WORKDIR /usr/src/app
COPY artifacts artifacts/

# set environment variables
ENV PYTHONDONTWRITEBYTECODE 1
ENV PYTHONUNBUFFERED 1

RUN pip install --upgrade pip
RUN pip --version
COPY load_testing/. app/.
RUN pip install -r app/requirements.txt

# copy project
COPY load_testing .