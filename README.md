

# RustSat

**RustSat** is a real-time satellite tracking software for Starlink and other Earth-orbiting satellites built with **Rust** and **Bevy**. It utilizes the SGP4 model for orbit propagation, providing real-time tracking and visualization of satellite positions.

## Key Features

- **SGP4 Orbit Propagation**: Computes precise satellite positions and velocities based on Rust SGP4 [crate](https://docs.rs/sgp4/latest/sgp4/).
- **Real-time TLE Updates**: Automatically retrieves updated satellite TLE data from online sources such as CelesTrak, with caching support for offline usage.
- **Plugin Based Design**: All functionalities are organized into plugin, allowing rapid adapation for various research purposes.
- **Position Visualization**: Provides accurate satellite position and velocity data visualized through Bevy.

## Installation and Setup

1. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/rustsat.git
   cd rustsat
   ```

2. Build the project:

   ```bash
   cargo build --release
   ```

3. Run RustSat:

   ```bash
   cargo run --release
   ```

## Usage

RustSat automatically loads TLE data from a local cache (`./tle.json`). If the cache is missing or outdated, RustSat fetches fresh TLE data online. Position updates and propagation are managed in real time, displaying latitude, longitude, and altitude for tracked satellites.

## Core Functionality

- **TLE Caching and Management**: RustSat first attempts to load TLE data from the local cache. If unavailable or outdated, it retrieves new data from online sources.
- **Orbit Propagation**: The `propagate_sat` function updates satellite positions in real time using the SGP4 model.
- **Coordinate Conversion**: Converts ECEF coordinates to geodetic (WGS84) format for accurate geographic positioning.

---

With **RustSat**, experience the power of Rust and Bevy for real-time satellite tracking!
![image](https://github.com/user-attachments/assets/ab9c65db-4464-4d95-87a4-5bf2f4630123)
## Citation
This software was initally developed for
[T. Cheng, T. Duan and V. Dinavahi, "Real-Time Cyber-Physical Digital Twin for Low Earth Orbit Satellite Constellation Network Enhanced Wide-Area Power Grid," in IEEE Open Journal of the Industrial Electronics Society, vol. 5, pp. 1029-1041, 2024, doi: 10.1109/OJIES.2024.3454010.](https://ieeexplore-ieee-org.login.ezproxy.library.ualberta.ca/document/10663871)
