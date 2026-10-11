import http
from fastapi import FastAPI, Path, Query
app = FastAPI(openapi_external_docs=external_docs)