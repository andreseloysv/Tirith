#!/usr/bin/env python3
"""Send OUR render (never the reference) to Gemini's image model on Vertex express mode with a lighting prompt.
python3 polish.py <in.png> <out-prefix> [model] [n]   (key: GOOGLE_CLOUD_API_KEY, sent only in the header)"""
import base64, json, os, sys, urllib.request

src, prefix = sys.argv[1], sys.argv[2]
model = sys.argv[3] if len(sys.argv) > 3 else 'gemini-2.5-flash-image'
n = int(sys.argv[4]) if len(sys.argv) > 4 else 1
PROMPT = (
    "Turn this render into a polished still from a high-end 3D animated feature film. Keep EXACTLY the same composition, "
    "camera angle, framing and every character's shape, size, position, colour and facial expression: do not add, remove or "
    "reshape any character, and keep the big yellow character round with its face where it is. Give every character a plump, soft, three-dimensional volume, like a glowing gummy or a plush lantern, with gentle shading that shows its roundness. Change only the rendering "
    "quality and the light: every character is a soft, slightly translucent body that glows from inside like a paper lantern, "
    "with a brighter warm core, subsurface scattering, softly glowing edges and a gentle bloom halo of its own colour. The big "
    "yellow character glows golden and saturated (not white), with a warm halo spilling into the space around it. Eyes stay "
    "glossy dark brown-black ovals with a tiny white catchlight; mouths stay open dark smiles with a red tongue. Background: "
    "deep space with blue and purple nebula and tiny stars, warm golden light behind the big character, fine glittering "
    "sparkle trails behind the small characters, a few soft out-of-focus sparkles (cinematic depth of field). Square image, "
    "no text, no watermark, no border."
)
img = base64.b64encode(open(src, 'rb').read()).decode()
body = {
    'contents': [{'role': 'user', 'parts': [{'inlineData': {'mimeType': 'image/png', 'data': img}}, {'text': PROMPT}]}],
    'generationConfig': {'responseModalities': ['IMAGE'], 'imageConfig': {'aspectRatio': '1:1'}},
}
url = f'https://aiplatform.googleapis.com/v1/publishers/google/models/{model}:generateContent'
for i in range(n):
    req = urllib.request.Request(url, data=json.dumps(body).encode(), method='POST',
                                 headers={'x-goog-api-key': os.environ['GOOGLE_CLOUD_API_KEY'], 'Content-Type': 'application/json'})
    try:
        d = json.load(urllib.request.urlopen(req, timeout=240))
    except urllib.error.HTTPError as e:
        print(model, 'HTTP', e.code, e.read()[:300].decode(errors='replace'))
        break
    parts = d.get('candidates', [{}])[0].get('content', {}).get('parts', [])
    pics = [p['inlineData'] for p in parts if 'inlineData' in p]
    if not pics:
        print(model, 'no image:', json.dumps(d)[:300])
        continue
    ext = 'png' if 'png' in pics[0].get('mimeType', '') else 'jpg'
    out = f'{prefix}-{model}-{i}.{ext}'
    open(out, 'wb').write(base64.b64decode(pics[0]['data']))
    print('WROTE', out, d.get('usageMetadata', {}).get('candidatesTokenCount'))
