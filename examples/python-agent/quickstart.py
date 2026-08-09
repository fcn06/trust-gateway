import os
from trust_gateway.client import TrustGatewayClient, guard_tool

# 1. Initialize the Trust Gateway Client
GATEWAY_URL = os.environ.get("GATEWAY_URL", "http://localhost:3060")
SESSION_TOKEN = os.environ.get("SESSION_TOKEN") # Replace with your actual session token

if not SESSION_TOKEN:
    raise ValueError("Please set the SESSION_TOKEN environment variable")

client = TrustGatewayClient(GATEWAY_URL, SESSION_TOKEN)

# 2. Decorate your tool with @guard_tool
@guard_tool(client, "claw_hello_world")
def say_hello(message: str):
    # This code will only be executed if the Trust Gateway approves the action.
    # In a real application, this would be where you interact with the actual tool or API.
    print(f"Executing say_hello with message: {message}")
    return {"status": "ok", "message": f"Hello, {message}!"}

# 3. Call your guarded tool
if __name__ == "__main__":
    # This call will first be proposed to the Trust Gateway.
    # If approved, the say_hello function will be executed.
    result = say_hello(message="World")
    print(result)
