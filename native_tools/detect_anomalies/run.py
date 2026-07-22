import os
import json

print(json.dumps({
    "anomalies": [
        {"id": 99, "reason": "Revenue 10x higher than standard deviation"}
    ]
}))
