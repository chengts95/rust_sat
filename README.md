

# RustSat

**RustSat** is a real-time satellite tracking software for Starlink and other Earth-orbiting satellites built with **Rust** and **Bevy**. It utilizes the SGP4 model for orbit propagation, providing real-time tracking and visualization of satellite positions.

## Key Features

- **SGP4 Orbit Propagation**: Computes precise satellite positions and velocities based on SGP4 (Simplified General Perturbations model).
- **Real-time TLE Updates**: Automatically retrieves updated satellite TLE data from online sources such as CelesTrak, with caching support for offline usage.
- **Multi-Satellite Tracking**: Supports simultaneous tracking of multiple satellites, including full Starlink constellation support.
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
