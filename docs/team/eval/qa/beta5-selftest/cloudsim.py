import os, sys, time, shutil
Q = sys.argv[1]; pool = sorted(os.listdir(Q + "/pool"))[:40]; cd = Q + "/sim/Library/CloudStorage/cloud"
for f in pool:
    tmp = os.path.join(cd, "." + f + ".part"); shutil.copyfile(os.path.join(Q, "pool", f), tmp)
    os.rename(tmp, os.path.join(cd, f)); os.remove(os.path.join(cd, "." + f + ".icloud")); time.sleep(1.2)
