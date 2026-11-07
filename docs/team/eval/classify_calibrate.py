"""Hiệu chỉnh confidence của classify (hồi quy logistic L2) trên split tune.
Dùng: classify_calibrate.py <lambda> <dump_tune.csv> <model.json>
dump_tune.csv sinh bởi `classify_eval --split tune --dump <file>` khi classify KHÔNG hạ class theo ngưỡng
(đặt tạm CONF_MIN = 0.0). Kết quả (mu/sd/w/b) dán vào CAL_* trong crates/fontatlas-core/src/classify.rs.
Lưu ý: bản gốc fit trên dump có cột pred/s_* của classifier trước khi hiệu chỉnh.
"""
import pandas as pd, numpy as np, sys, json
d=pd.read_csv(sys.argv[2])
d=d[d.pred.isin(["Serif","Sans","Script","Mono"]) & d.accept.notna()].reset_index(drop=True)
d["ok"]=[p in a.split("|") for p,a in zip(d.pred,d.accept)]
CL=["Serif","Sans","Script","Mono"]
SC={"Serif":"s_serif","Sans":"s_sans","Script":"s_script","Mono":"s_mono"}  # s_display = bằng chứng nổi bật (vote None)
def clamp(x,a,b,miss):
    x=np.where(np.isnan(x),miss,x); return np.clip(x,a,b)
def feats(d):
    cols=[]; names=[]
    for c in CL:
        cols.append((d.pred==c).astype(float).values); names.append("is_"+c)
    S=np.stack([d[SC[c]].values for c in CL],1)
    idx=np.array([CL.index(p) if p in CL else 0 for p in d.pred])
    own=S[np.arange(len(d)),idx]
    S2=S.copy(); S2[np.arange(len(d)),idx]=-9
    margin=np.clip(own-S2.max(1),-2,5)
    cols.append(margin); names.append("margin")
    cols.append(np.where(d.pred=="Script",0,np.clip(d.s_display.values,0,4))); names.append("odd_display")
    cols.append(np.where(d.pred=="Script",0,np.clip(d.s_script.values,0,4))); names.append("odd_script")
    cols.append(clamp(d.width_cv.values,0,.5,.25)); names.append("width_cv")
    cols.append(clamp(d.spacing.values,0,.4,.15)); names.append("spacing")
    cols.append(clamp(d.aperture_e.values,0,.4,.2)); names.append("aperture_e")
    cols.append(clamp(d.counter.values,0,.7,.45)); names.append("counter")
    cols.append(np.log(clamp(d.contrast.values,1,8,1.5))); names.append("ln_contrast")
    cols.append(clamp(d.terminal.values,0,3,1.2)); names.append("terminal")
    cols.append(clamp(d.bounce.values,0,.2,.015)); names.append("bounce")
    cols.append(clamp(d.connect.values,0,1,.2)); names.append("connect")
    return np.stack(cols,1),names
X,names=feats(d); y=d.ok.values.astype(float)
mu=X.mean(0); sd=X.std(0)+1e-9
# one-hots: keep unscaled
for i in range(4): mu[i]=0; sd[i]=1
Z=(X-mu)/sd
def fit(Z,y,lam):
    w=np.zeros(Z.shape[1]); b=0.0
    for _ in range(3000):
        p=1/(1+np.exp(-(Z@w+b)))
        g=Z.T@(p-y)/len(y)+lam*w; gb=(p-y).mean()
        w-=0.5*g; b-=0.5*gb
    return w,b
lam=float(sys.argv[1]) if len(sys.argv)>1 else 0.01
w,b=fit(Z,y,lam)
p=1/(1+np.exp(-(Z@w+b)))
# 5-fold CV precision/coverage on tune
rng=np.random.default_rng(0); idx=rng.permutation(len(y)); folds=np.array_split(idx,5); cv=np.zeros(len(y))
for f in folds:
    tr=np.setdiff1d(idx,f); w2,b2=fit(Z[tr],y[tr],lam); cv[f]=1/(1+np.exp(-(Z[f]@w2+b2)))
assigned=np.ones(len(d),bool)
print("lam",lam,"n",len(y),"base prec",y[assigned].mean())
for th in [.5,.6,.7,.75,.8,.85,.9,.93]:
    m=assigned&(p>=th); mc=assigned&(cv>=th)
    print(th,"fit cov %.3f prec %.3f | cv cov %.3f prec %.3f"%(m.mean(),y[m].mean() if m.any() else np.nan,mc.mean(),y[mc].mean() if mc.any() else np.nan))
json.dump({"names":names,"mu":mu.tolist(),"sd":sd.tolist(),"w":w.tolist(),"b":b},open(sys.argv[3],"w"))
