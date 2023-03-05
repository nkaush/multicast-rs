#!/usr/bin/env bash

folder=$1
count=${folder:0:1}

mkdir -p $folder/tx/
mkdir -p $folder/logs/
for ((i=1; i<=$count; i++))
do
   scp "neilk3@sp23-cs425-450$i.cs.illinois.edu":/home/neilk3/mp1/transactions.log `pwd`"/$folder/tx/transactions-$i.log" &
   scp "neilk3@sp23-cs425-450$i.cs.illinois.edu":/home/neilk3/mp1/latencies.log `pwd`"/$folder/latencies-$i.log" &
   scp "neilk3@sp23-cs425-450$i.cs.illinois.edu":/home/neilk3/mp1/log.log `pwd`"/$folder/logs/log-$i.log" &
done

wait

python3 plot.py $folder