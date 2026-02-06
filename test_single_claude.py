import requests
import json
import time

url = "http://localhost:8045/v1/messages"
headers = {
    "Content-Type": "application/json",
    "x-api-key": "sk-79754f1fecab4451aa8bb9f0ba4d5ee9", 
    "anthropic-version": "2023-06-01"
}
# 用户请求: "4k 改成 1k standard"
# size: "1k" -> 映射为 imageSize: "1024x1024" (如果不识别则为默认)
# quality: "standard" -> 映射为 quality: "standard"
data = {
    "model": "gemini-3-pro-image",
    "messages": [
        {"role": "user", "content": "A futuristic city with flying cars, cinematic lighting, 8k"}
    ],
    "max_tokens": 1024,
    "size": "1k", 
    "quality": "standard"
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
