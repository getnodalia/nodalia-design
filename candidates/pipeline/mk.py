import json
COL={'forja':['#D9976A','#A9622F'],'trama':['#9A45E2','#7A26C9'],'vinilo':['#A87BFF','#5A2BD8'],'pliego':['#7088FF','#3F5BE0'],'hito':['#33C48A','#1B8F5A']}
T={
'forja-eiffel':('forja-eiffel.jpg',[447,468,1740,1762],{}),
'forja-brunel':('forja-brunel.jpg',[208,61,2051,1904],{}),
'forja-nasmyth':('forja-nasmyth.tif',[682,-55,4094,3357],{'gamma':0.7}),
'forja-tesla':('forja-tesla.jpg',[-234,80,3736,4051],{}),
'trama-morris':('trama-morris.jpg',[-108,38,1404,1550],{'fade':90}),
'trama-lovelace':('trama-lovelace-b.jpg',[322,163,2491,2333],{}),
'trama-babbage':('trama-babbage.jpg',[656,362,2797,2503],{}),
'trama-jacquard':('trama-jacquard-b.jpeg',[0,157,2218,2375],{'pre':[40,776,2258,3463],'fade':100}),
'vinilo-berliner':('vinilo-berliner.tif',[134,186,3697,3749],{}),
'vinilo-caruso':('vinilo-caruso.jpg',[-63,31,2728,2822],{}),
'vinilo-beethoven':('vinilo-beethoven-a.jpg',[628,-231,4330,3471],{'fade':110}),
'vinilo-edison':('vinilo-edison-b.jpg',[780,32,2392,1643],{'fade':110}),
'pliego-franklin':('pliego-franklin.jpg',[-92,257,2908,3257],{}),
'pliego-baskerville':('pliego-baskerville.jpg',[241,42,2001,1802],{'fade':110}),
'pliego-mergenthaler':('pliego-mergenthaler-b.jpg',[-260,-162,2913,3011],{'blur':2.5}),
'hito-taylor':('hito-taylor.jpg',[461,-113,2344,1770],{'fade':110}),
'hito-taylor-b':('hito-taylor-b.jpg',[177,-137,2339,2025],{'pre':[0,0,2420,2330]}),
'hito-taylor-c':('hito-taylor-c.jpg',[174,20,1294,1139],{}),
'hito-gilbreth':('hito-gilbreth.jpg',[320,39,2514,2233],{}),
}
cfg={}
for n,(f,box,extra) in T.items():
    cfg[n]=dict(file=f,box=box,colors=COL[n.split('-')[0]],**extra)
json.dump(cfg,open('figures.json','w'),indent=1)
