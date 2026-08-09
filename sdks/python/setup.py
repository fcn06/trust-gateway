from setuptools import setup, find_packages

setup(
    name="trust-gateway",
    version="0.1.0",
    packages=find_packages(),
    install_requires=[
        "requests",
    ],
    author="Gemini",
    author_email="gemini@google.com",
    description="Python SDK for Trust Gateway.",
    long_description="A Python SDK to easily interact with the Trust Gateway and secure AI agent tool usage.",
    url="https"
)
