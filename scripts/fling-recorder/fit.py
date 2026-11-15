import json, math, sys
INF, ST, ET, N = 0.35, 0.5, 1.0, 100
P1, P2 = ST*INF, 1-ET*(1-INF)
def table():
    pos=[0.0]*(N+1); xmin=0.0
    for i in range(N):
        a=i/N; xmax=1.0
        while True:
            x=xmin+(xmax-xmin)/2; c=3*x*(1-x); tx=c*((1-x)*P1+x*P2)+x**3
            if abs(tx-a)<1e-5: break
            if tx>a: xmax=x
            else: xmin=x
        pos[i]=c*((1-x)*ST+x)+x**3
    pos[N]=1.0; return pos
POS=table()
def spline(u):
    if u>=1: return 1.0
    i=int(N*u); ti=i/N; v=(POS[i+1]-POS[i])*N; return POS[i]+(u-ti)*v
G=9.80665; TUNE=G*39.37*160*0.84; FR=0.015; DR=2.3582018
def dur(v): l=math.log(INF*abs(v)/(FR*TUNE)); return math.exp(l/(DR-1))
def dist(v): l=math.log(INF*abs(v)/(FR*TUNE)); return FR*TUNE*math.exp(DR/(DR-1)*l)
def fit(path):
    d=json.load(open(path)); ev=d['events']
    tend=[e['t'] for e in ev if e['kind'] in ('touchend','touchcancel')][-1]
    fr=[(e['t'],e['top']) for e in ev if e['kind']=='frame' and e['t']>tend]
    sign=1 if fr[-1][1]>fr[0][1] else -1
    best=None
    for k in range(0,4):  # fling starts at one of the first frames after up
        t0,p0=fr[k]
        rest=fr[k:]
        total=(fr[-1][1]-p0)*sign
        if total<=0: continue
        # v from total distance
        lo,hi=1.0,40000.0
        for _ in range(80):
            m=(lo+hi)/2
            if dist(m)<total: lo=m
            else: hi=m
        v=lo
        for dt0 in [x*0.5 for x in range(-40,41)]:
            T=dur(v); err=0
            for t,top in rest:
                u=max(0,(t-(t0+dt0))/1000/T)
                pred=p0+sign*dist(v)*spline(u)
                err=max(err,abs(pred-top))
            if best is None or err<best[0]: best=(err,k,dt0,v,T,total)
    err,k,dt0,v,T,total=best
    moving=[t for (t,top),(t2,top2) in zip(fr,fr[1:]) if top2!=top]
    print(f"{path.split('/')[-1]}: v {v:.0f} dip/s, curve {T*1000:.0f} ms over {total:.1f} px; recorded coast {moving[-1]-fr[k][0]+16.7 if moving else 0:.0f} ms; worst frame off {err:.2f} px (start frame {k}, t0 shift {dt0} ms)")
for p in sys.argv[1:]: fit(p)

def drag_velocity(path):
    d=json.load(open(path)); ev=d['events']
    tend=[e['t'] for e in ev if e['kind'] in ('touchend','touchcancel')][-1]
    fr=[(e['t'],e['top']) for e in ev if e['kind']=='frame' and e['t']<=tend]
    if len(fr)<3: return None
    (ta,pa),(tb,pb)=fr[-3],fr[-1]
    return (pb-pa)/(tb-ta)*1000
def last_velocity(path):
    d=json.load(open(path)); fr=[(e['t'],e['top']) for e in d['events'] if e['kind']=='frame']
    (ta,pa),(tb,pb)=fr[-3],fr[-1]
    return (pb-pa)/(tb-ta)*1000

def fit_partial(path, frames_used=12):
    """v and t0 fitted to the first frames of the coast alone (for a
    fling cut by the next touch or the list's end)."""
    d=json.load(open(path)); ev=d['events']
    tend=[e['t'] for e in ev if e['kind'] in ('touchend','touchcancel')][-1]
    fr=[(e['t'],e['top']) for e in ev if e['kind']=='frame' and e['t']>tend]
    sign=1 if fr[min(5,len(fr)-1)][1]>fr[0][1] else -1
    best=None
    for k in range(1,4):
        t0,p0=fr[k]; rest=fr[k:k+frames_used]
        for v in range(300,12000,25):
            T=dur(v); D=dist(v)
            for dt0 in range(-20,21,2):
                err=max(abs(p0+sign*D*spline(max(0,(t-(t0+dt0))/1000/T))-top) for t,top in rest)
                if best is None or err<best[0]: best=(err,k,v,dt0)
    return best
