import json,sys,statistics as st
S=sys.argv[1]
rows=[l.rstrip('\n').split('\t') for l in open(f'{S}/stress_set.tsv')]
txt=open(f'{S}/gf_dump.json').read(); dec=json.JSONDecoder(); i=0; dumps=[]
while i<len(txt):
    while i<len(txt) and txt[i].isspace(): i+=1
    if i>=len(txt): break
    o,i=dec.raw_decode(txt,i); dumps.append(o)
def find(d,k):
    if isinstance(d,dict):
        if k in d: return d[k]
        for v in d.values():
            r=find(v,k)
            if r is not None: return r
assert len(dumps)==len(rows),(len(dumps),len(rows))
res={}
fails={}
for (fam,g,_),d in zip(rows,dumps):
    face=d['faces'][0]; s=find(face,'stress_deg'); c=find(face,'contrast')
    if s and s.get('value') is not None: res.setdefault(g,[]).append((s['value'],fam,c.get('value') if c else None))
    else: fails.setdefault(g,[]).append((fam,(s or {}).get('quality')))
for g in ('vertical','transitional','oblique'):
    v=sorted(x[0] for x in res.get(g,[]))
    n=len(v); f=len(fails.get(g,[]))
    q=lambda p: v[min(n-1,int(p*n))]
    print(f"{g:13} n={n:3} failed={f:3}  p10={q(.1):5} median={st.median(v):5} p90={q(.9):5}  |>=5°|: {sum(abs(x)>=5 for x in v)}/{n}")
# simple classifier: vertical vs oblique by threshold
V=[x[0] for x in res['vertical']]; O=[x[0] for x in res['oblique']]
for t in (3,5,7,9):
    acc=(sum(abs(x)<t for x in V)+sum(abs(x)>=t for x in O))/(len(V)+len(O))
    print(f"threshold {t}°: vertical<t {sum(abs(x)<t for x in V)}/{len(V)}, oblique>=t {sum(abs(x)>=t for x in O)}/{len(O)}, acc {acc:.2f}")
print('neg oblique', sum(x<0 for x in O))
print('vertical outliers', sorted([(x[0],x[1]) for x in res['vertical'] if abs(x[0])>=7])[:12])
print('oblique near0', sorted([(x[0],x[1]) for x in res['oblique'] if abs(x[0])<3])[:20])
print('fail reasons', {g:[str(q) for _,q in v][:5] for g,v in fails.items()})
