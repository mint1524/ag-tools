import requests
import json
import time

BASE_URL = "http://localhost:8045"
API_KEY = "sk-79754f1fecab4451aa8bb9f0ba4d5ee9"

HEADERS = {
    "Authorization": f"Bearer {API_KEY}",
    "Content-Type": "application/json"
}

def test_openai_protocol():
    print("\n--- Testing OpenAI Protocol (/v1/images/generations) ---")
    url = f"{BASE_URL}/v1/images/generations"
    data = {
        "model": "gemini-3-pro-image",
        "prompt": "A cute robot, minimalist style",
        "size": "16:9",
        "quality": "hd",
        "n": 1
    }
    
    try:
        start = time.time()
        # Mocking or expecting error? Real generation takes time.
        # We might just want to see if it accepts parameters without 400 error 
        # or if we can see expected mapping in server logs.
        # But let's try to send it.
        print(f"Sending request: {json.dumps(data)}")
        response = requests.post(url, headers=HEADERS, json=data, timeout=30)
        print(f"Status: {response.status_code}")
        if response.status_code == 200:
            print("Success! (Partial response content hidden)")
            # print(response.json()) 
        else:
            print(f"Error: {response.text}")
    except Exception as e:
        print(f"Request failed: {e}")

def test_claude_protocol():
    print("\n--- Testing Claude Protocol (/v1/messages) ---")
    url = f"{BASE_URL}/v1/messages"
    # Claude headers typically require x-api-key not Authorization, but proxy supports Bearer too usually?
    # Antigravity Proxy mapping `v1/messages` usually expects Anthropic format.
    # We'll use standard Anthropic headers just in case.
    claude_headers = {
        "x-api-key": API_KEY,
        "anthropic-version": "2023-06-01",
        "content-type": "application/json"
    }
    
    data = {
        "model": "gemini-3-pro-image",
        "messages": [
            {"role": "user", "content": "A cute robot, minimalist style"}
        ],
        "max_tokens": 1024,
        # Claude custom parameters for image gen (supported by our proxy)
        "size": "1:1",
        "quality": "standard"
    }
    
    try:
        print(f"Sending request: {json.dumps(data)}")
        response = requests.post(url, headers=claude_headers, json=data, timeout=30)
        print(f"Status: {response.status_code}")
        if response.status_code == 200:
             print("Success!")
        else:
             print(f"Error: {response.text}")
    except Exception as e:
        print(f"Request failed: {e}")

def test_gemini_native_protocol():
    print("\n--- Testing Gemini Native Protocol (/v1beta/models/...:generateContent) ---")
    # Note: Antigravity Proxy typically maps `/google/v1beta/...` or similar.
    # Or standard `v1beta` if routed.
    # Let's assume standard google path is supported on root or via /google prefix.
    # The user asked to test "3 protocols", usually implies OpenAI, Claude, and Google.
    
    url = f"{BASE_URL}/v1beta/models/gemini-3-pro-image:generateContent?key={API_KEY}"
    
    data = {
        "contents": [{
            "parts": [{"text": "A cute robot, minimalist style"}]
        }],
        "generationConfig": {
            "imageConfig": {
                "aspectRatio": "4:3",
                "imageSize": "2K"
            }
        }
    }
    
    try:
        print(f"Sending request: {json.dumps(data)}")
        response = requests.post(url, headers={"Content-Type": "application/json"}, json=data, timeout=30)
        print(f"Status: {response.status_code}")
        if response.status_code == 200:
             print("Success!")
        else:
             print(f"Error: {response.text}")
    except Exception as e:
        print(f"Request failed: {e}")

if __name__ == "__main__":
    test_openai_protocol()
    test_claude_protocol()
    test_gemini_native_protocol()
