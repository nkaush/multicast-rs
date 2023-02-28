import matplotlib.pyplot as plt
from pandas import DataFrame
from numpy import arange
import glob, sys, os

max_tx_latencies = {}
data_path = sys.argv[1]
out_file = os.path.join(data_path, 'cdf.png')

for filename in glob.glob(os.path.join(data_path, '*.log')):
    with open(os.path.join(os.getcwd(), filename), 'r') as f: 
        for l in f.readlines():
            tx_id, latency = l.split(',')
            latency = float(latency)
            if tx_id in max_tx_latencies:
                max_tx_latencies[tx_id] = max(max_tx_latencies[tx_id], latency)
            else:
                max_tx_latencies[tx_id] = latency

step=0.001
indices = arange(step,1,step)
latencies = max_tx_latencies.values()
cdf = DataFrame({'latencies': latencies})['latencies'].quantile(indices)

indices *= 100

plt.plot(cdf, indices, linewidth=2, label='latency', color='blue')
plt.legend(loc='best')
plt.savefig(out_file)