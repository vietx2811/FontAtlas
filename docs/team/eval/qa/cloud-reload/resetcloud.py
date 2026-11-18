import os, shutil, sys
Q = sys.argv[1]; pool = sorted(os.listdir(Q + '/pool-cloud')); cd = Q + '/cloud'; shutil.rmtree(cd, ignore_errors=True); os.makedirs(cd)
for i, f in enumerate(pool):
    if i <= 30: open(os.path.join(cd, '.' + f + '.icloud'), 'wb').write(b'bplist00-stub')
    else: open(os.path.join(cd, f), 'wb').close()
