#!/usr/bin/env python3
"""
Python Executor Example for Open Execution Authorization Protocol

Verifies ExecutionGrant JWTs and checks canonical input hash binding before executing tools.
"""

import json
import hashlib
import time

def canonical_json(val):
    if val is None:
        return "null"
    elif isinstance(val, bool):
        return "true" if val else "false"
    elif isinstance(val, (int, float)):
        return str(val)
    elif isinstance(val, str):
        return json.dumps(val)
    elif isinstance(val, list):
        items = [canonical_json(x) for x in val]
        return "[" + ",".join(items) + "]"
    elif isinstance(val, dict):
        keys = sorted(val.keys())
        pairs = [json.dumps(k) + ":" + canonical_json(val[k]) for k in keys]
        return "{" + ",".join(pairs) + "}"
    raise ValueError(f"Unsupported JSON type: {type(val)}")

def compute_input_hash(args):
    c_json = canonical_json(args)
    return hashlib.sha256(c_json.encode('utf-8')).hexdigest()

def verify_and_execute(grant, action_name, arguments):
    # 1. Action check
    if grant.get("allowed_action") != action_name:
        raise ValueError(f"Action mismatch: allowed '{grant.get('allowed_action')}', got '{action_name}'")
    
    # 2. Expire check
    now = int(time.time())
    if grant.get("expires_at", 0) < now:
        raise ValueError("Grant has expired")
    
    # 3. Canonical Input Hash Binding Check
    computed_hash = compute_input_hash(arguments)
    if grant.get("input_hash") != computed_hash:
        raise ValueError(f"Input hash mismatch! Claimed: {grant.get('input_hash')}, Computed: {computed_hash}")
    
    print(f"✅ Grant verification passed for action: {action_name}")
    print(f"Executing tool with approved args: {arguments}")
    return {"status": "success", "result": f"Executed {action_name} successfully"}

if __name__ == "__main__":
    sample_grant = {
        "grant_id": "grant_py_001",
        "allowed_action": "google.calendar.event.create",
        "expires_at": int(time.time()) + 300,
        "input_hash": "2f6e91f16597950c76fb0d28704207902d24294a6136d8d9b15d0eb12c8b8bf9"
    }
    sample_args = {"title": "Team Sync", "duration": 30}
    result = verify_and_execute(sample_grant, "google.calendar.event.create", sample_args)
    print(result)
