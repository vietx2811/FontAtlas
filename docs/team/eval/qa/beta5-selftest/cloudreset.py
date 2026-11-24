import os, sys, shutil
Q = sys.argv[1]; pool = sorted(os.listdir(Q + "/pool"))[:40]; cd = Q + "/sim/Library/CloudStorage/cloud"
shutil.rmtree(cd, ignore_errors=True); os.makedirs(cd)
for f in pool: open(os.path.join(cd, "." + f + ".icloud"), "wb").write(b"bplist00-stub")
print(len(pool), "stubs")
