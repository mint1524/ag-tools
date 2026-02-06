import requests
import json
import time

# Gemini Native Endpoint (via Proxy)
url = "http://localhost:8045/v1beta/models/gemini-3-pro-image:generateContent?key=sk-79754f1fecab4451aa8bb9f0ba4d5ee9"
headers = {
    "Content-Type": "application/json"
}

# Native Payload Structure
data = {
    "contents": [
        {
            "parts": [
                {"text": "A futuristic city with flying cars, cinematic lighting, 8k"}
            ]
        }
    ],
    "generationConfig": {
        "imageConfig": {
            "aspectRatio": "16:9"
            # "imageSize": "2K"
        }
    }
}

print(f"Sending request to {url}...")
print(f"Data: {json.dumps(data, indent=2)}")

try:
    start_time = time.time()
    response = requests.post(url, headers=headers, json=data, timeout=60)
    elapsed = time.time() - start_time
    
    print(f"\nResponse received in {elapsed:.2f}s")
    print(f"Status Code: {response.status_code}")
    print(f"Response Body: {response.text}")
    
except requests.exceptions.RequestException as e:
    print(f"\nRequest failed: {e}")
