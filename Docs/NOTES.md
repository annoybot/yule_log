# ULOG FIle format Specifications

* [UlogFile Format](https://docs.px4.io/main/en/dev_log/ulog_file_format.html)
* [PX4 Devguide](https://github.com/PX4/PX4-Devguide/blob/master/en/log/ulog_file_format.md)

# Useful Commands

## XSV: Remove columns

PlotJuggler does not emit the following columns for some reason.  This cmdline makes 
it easy to remove them from the output of `ulog2csv` to make it easier to comapre results.

```shell
xsv select '__time-battery_status.00/state_of_health,battery_status.00/voltage_cell_v.00-battery_status.01/state_of_health,battery_status.01/voltage_cell_v.00-estimator_innovation_test_ratios/aux_hvel.01,estimator_innovation_test_ratios/baro_vpos-estimator_innovation_test_ratios/ev_vvel,estimator_innovation_test_ratios/flow.00-estimator_innovation_variances/aux_hvel.01,estimator_innovation_variances/baro_vpos-estimator_innovation_variances/ev_vvel,estimator_innovation_variances/flow.00-estimator_innovations/aux_hvel.01,estimator_innovations/baro_vpos-estimator_innovations/ev_vvel,estimator_innovations/flow.00-position_setpoint_triplet/current/landing_gear,position_setpoint_triplet/current/loiter_direction-position_setpoint_triplet/current/loiter_radius,position_setpoint_triplet/current/pitch_min-position_setpoint_triplet/next/landing_gear,position_setpoint_triplet/next/loiter_direction-position_setpoint_triplet/next/loiter_radius,position_setpoint_triplet/next/pitch_min-position_setpoint_triplet/previous/landing_gear,position_setpoint_triplet/previous/loiter_direction-position_setpoint_triplet/previous/loiter_radius,position_setpoint_triplet/previous/pitch_min-sensor_mag.00/is_external,sensor_mag.00/timestamp_sample-sensor_mag.01/is_external,sensor_mag.01/timestamp_sample-vehicle_gps_position/hdop,vehicle_gps_position/heading_offset-vehicle_imu_status.02/gyro_vibration_metric,vehicle_land_detected/freefall-vehicle_local_position/evv,vehicle_local_position/heading-vehicle_local_position/vx,vehicle_local_position/vxy_reset_counter-vehicle_local_position/vz,vehicle_local_position/vz_reset_counter-yaw_estimator_status/yaw_variance' sample_log_small_export.csv > out.csv
```

# Mysql

## Database setup

```mysql
-- Create the database if it doesn't already exist
CREATE DATABASE IF NOT EXISTS ulog;

-- Create the user if it doesn't exist, and set its password to 'password'
CREATE USER IF NOT EXISTS 'ulog'@'localhost' IDENTIFIED BY 'password';

-- If the user already exists, update the password
ALTER USER 'ulog'@'localhost' IDENTIFIED BY 'password';

-- Grant all privileges on the 'ulog' database to the 'ulog' user
GRANT ALL PRIVILEGES ON ulog.* TO 'ulog'@'localhost';

-- Flush privileges to ensure that changes are applied
FLUSH PRIVILEGES;

```