import os
import json

args = json.loads(os.getenv("SKILL_ARGS", "{}"))
count = args.get("count", 5)
print(json.dumps({
    "rows": [
        {"id": 1, "revenue": 1200.50, "category": "SaaS"},
        {"id": 2, "revenue": 450.00, "category": "Hardware"}
    ]
}))
