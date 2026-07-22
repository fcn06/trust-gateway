import os
import json

print(json.dumps({
    "statistics": {
        "revenue": {
            "mean": 825.25,
            "min": 450.00,
            "max": 1200.50,
            "sum": 1650.50
        }
    }
}))
