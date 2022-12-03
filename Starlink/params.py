import xlrd
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

# parameters
R = 6371                # average radius of earth, 6371km
start_longitude = -114.029 # start point -- Calgary, jingdu
start_latitude = 50.826    # start point, weidu
end_longitude = -79.25   # end point -- Toronto
end_latitude = 43.40     # end point, weidu

'''read starlink data'''
csv_data = pd.read_csv('test1.csv')
ID_data = csv_data['EntityID'].values
X_data = csv_data['TemeCoord1'].values
Y_data = csv_data['TemeCoord2'].values
Z_data = csv_data['TemeCoord3'].values
Latitude_data = csv_data['Latitude'].values
Longitude_data = csv_data['Longitude'].values
Altitude_data = csv_data['Altitude'].values
num_rows = np.shape(ID_data)[0]
print(num_rows)

'''Plot Results'''
# the load of each day in one year
'''day_366 = range(1,367)
fig1 = plt.figure(1)
plt.plot(day_366,load_per_day_high,day_366,load_per_day_low)
plt.title('Load_per_day')
plt.ylabel('load(MW)')
plt.xlabel('day')
# the load of each hour in one day
fig2 = plt.figure(2)
plt.plot(load_per_hour[:,3])
plt.show()'''