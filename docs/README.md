# 📊 ESP32S3-BOX-Lite Display and Publish

Display real-time sensor data from a BME680 sensor on the ESP32S3-BOX-Lite device and publish it using no_std Rust! 🦀

![Sensor data displayed](images/display_sensor_data.jpg)

📚 Using functionality from my other project: [esp32s3 no_std Async TLS MQTT](https://github.com/sambenko/esp32s3-no-std-async-tls-mqtt)

---

## 📋 Table of Contents

- [🎯 About The Project](#-about-the-project)
- [🎨 Graphical Crates](#-graphical-crates)
- [🔧 Prerequisites and Getting Started](#-prerequisites-and-getting-started)
  - [Hardware Specific to This Project](#hardware-specific-to-this-project)


---

## 🎯 About The Project

This project extends upon the previous [esp32s3 no_std Async TLS MQTT](https://github.com/sambenko/esp32s3-no-std-async-tls-mqtt) to utilize the display of ESP32S3-BOX-Lite and showing real-time data from a BME680 sensor 🌡. Measurements of Temperature, Humidity and Gas Resistance are displayed and are updated every 60 seconds.

[🔝 back to top](#-table-of-contents)

---

## 🎨 Graphical Crates

- [mipidsi](https://github.com/almindor/mipidsi) for the display drivers 🖥
- [esp-box-ui](https://github.com/sambenko/esp-box-ui) for UI elements 🎨

[🔝 back to top](#-table-of-contents)

---

## 🔧 Prerequisites and Getting Started

### Hardware Specific to This Project

- ESP32S3-BOX-Lite devkit 🛠
- BME680 environmental sensor 🌡

For Software Requirements, Hardware Setup, Setting up MQTT, secrets/ folder and Running the Program, please refer to the corresponding sections in the [esp32s3 no_std Async TLS MQTT](https://github.com/sambenko/esp32s3-no-std-async-tls-mqtt).



[🔝 back to top](#-table-of-contents)

