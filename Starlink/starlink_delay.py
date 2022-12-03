import numpy as np
import pandas as pd
from math import radians, cos, sin, asin, sqrt, pi
from params import *

# the distance between two nodes
def geodistance(R,position1,position2):
    lng1 = position1.longitude
    lat1 = position1.latitude
    lng2 = position2.longitude
    lat2 = position2.latitude
    # transform degree to radian
    lng1, lat1, lng2, lat2 = map(radians, [float(lng1), float(lat1), float(lng2), float(lat2)])
    dlon = lng2-lng1
    dlat = lat2-lat1
    # a is actually equal to haversin(d/2R), i.e., sin2(d/2R)
    a = sin(dlat/2)**2 + cos(lat1) * cos(lat2) * sin(dlon/2)**2
    # asin: arcsin()
    distance = 2*asin(sqrt(a))*R
    # contain 3 numbers after dot
    distance = round(distance,3)
    # return results, km
    return distance

# the radian degree between two nodes
def geodegree(R,position1,position2):
    lng1 = position1.longitude
    lat1 = position1.latitude
    lng2 = position2.longitude
    lat2 = position2.latitude
    # transform degree to radian
    lng1, lat1, lng2, lat2 = map(radians, [float(lng1), float(lat1), float(lng2), float(lat2)])
    dlon = lng2-lng1
    dlat = lat2-lat1
    # a is actually equal to haversin(d/2R), i.e., sin2(d/2R)
    a = sin(dlat/2)**2 + cos(lat1) * cos(lat2) * sin(dlon/2)**2
    # asin: arcsin()
    degree = 2*asin(sqrt(a))
    # return results
    return degree

# given the two side lengths and their theta_radian, compute the third length
def trangle_distance(A,B,theta):
    C = sqrt(A**2 + B**2 - 2*A*B*cos(theta))
    return C

# find the distance between GS and satellite
def distance_GS_satelite(GS_position, Sate_position, H, R):
    theta_radians = geodegree(R,GS_position,Sate_position)
    sigma_radians = 0
    d = R*(sqrt(((H+R)/R)**2-cos(sigma_radians)**2)-sin(sigma_radians))
    H_GS = d*sin(sigma_radians)
    D_GS_Sate = trangle_distance(R+H,R+H_GS,theta_radians)
    return D_GS_Sate

class geoposition:
    def __init__(self,input1,input2):
        self.longitude = input1     # longitude, unit is degree
        self.latitude = input2     # latitude, unit is degree

start_point = geoposition(start_longitude,start_latitude)
end_point = geoposition(end_longitude,end_latitude)

distance_start_storage = np.zeros(num_rows)
distance_end_storage = np.zeros(num_rows)
for sate_num in range(0,int(num_rows/1)):
    # get the starlink position
    star_node = geoposition(0,0)
    star_node.longitude = Longitude_data[sate_num]
    star_node.latitude = Latitude_data[sate_num]
    star_node_altitude = Altitude_data[sate_num]
    # print(star_node_altitude)
    # get the distance between GS and starlink node
    distance_start_storage[sate_num] = distance_GS_satelite(start_point, star_node, star_node_altitude, R)
    distance_end_storage[sate_num] = distance_GS_satelite(end_point, star_node, star_node_altitude, R)


# get the minimized distance starlink
start_min_num = np.argmin(distance_start_storage)
end_min_num = np.argmin(distance_end_storage)
# get the sigma degree
start_sigma_degree = asin( (Altitude_data[start_min_num]*(Altitude_data[start_min_num]+2*R)-
                            distance_start_storage[start_min_num]**2)/(2*distance_start_storage[start_min_num]*R) )
end_sigma_degree = asin( (Altitude_data[end_min_num]*(Altitude_data[end_min_num]+2*R)-
                            distance_end_storage[end_min_num]**2)/(2*distance_end_storage[end_min_num]*R) )
print(start_sigma_degree)
print(end_sigma_degree)
# get the distance between two starlink satellite
print(start_min_num)
print(end_min_num)
distance_xx = sqrt((X_data[start_min_num]-X_data[end_min_num])**2+(Y_data[start_min_num]-Y_data[end_min_num])**2+(Z_data[start_min_num]-Z_data[end_min_num])**2)
print(distance_xx)
print(distance_start_storage.min())
print(distance_end_storage.min())

'''data_csv = pd.DataFrame(distance_start_storage)
data_csv.to_csv('results/distance_storage.csv')'''