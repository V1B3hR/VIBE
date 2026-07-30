import sys

try:
    with open('build_errors_3.txt', 'rb') as f:
        content = f.read()
    
    # Try different encodings
    for encoding in ['utf-16', 'utf-16-le', 'utf-8']:
        try:
            text = content.decode(encoding)
            print(f"--- Decoded with {encoding} ---")
            print(text)
            break
        except:
            continue
except Exception as e:
    print(f"Error: {e}")
