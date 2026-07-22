import os
import json

args = json.loads(os.getenv("SKILL_ARGS", "{}"))
dataset = args.get("dataset", "unknown")
print(json.dumps({
    "dataset": dataset,
    "columns": [
        {"name": "id", "type": "integer"},
        {"name": "revenue", "type": "float"},
        {"name": "category", "type": "string"},
        {"name": "timestamp", "type": "timestamp"}
    ]
}))
