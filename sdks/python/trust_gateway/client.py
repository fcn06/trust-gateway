
import requests
import functools
import json

class TrustGatewayClient:
    def __init__(self, gateway_url, session_token):
        self.gateway_url = gateway_url
        self.session_token = session_token

    def propose_action(self, tool_name, **kwargs):
        headers = {
            "Authorization": f"Bearer {self.session_token}",
            "Content-Type": "application/json"
        }
        payload = {
            "action_name": tool_name,
            "arguments": kwargs
        }
        response = requests.post(f"{self.gateway_url}/v1/actions/propose", headers=headers, json=payload)
        response.raise_for_status()
        return response.json()

def guard_tool(client, tool_name):
    def decorator(func):
        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            try:
                response = client.propose_action(tool_name, **kwargs)
                if response.get("status") == "succeeded":
                    print(f"Trust Gateway approved action: {tool_name}")
                    # In a real scenario, the executor would verify the grant
                    # and then execute the tool. For this SDK, we'll just
                    # print the result.
                    return response.get("result")
                else:
                    print(f"Trust Gateway denied action: {tool_name}")
                    return {"error": "Action denied by Trust Gateway"}
            except requests.exceptions.RequestException as e:
                print(f"Error communicating with Trust Gateway: {e}")
                return {"error": str(e)}
        return wrapper
    return decorator
