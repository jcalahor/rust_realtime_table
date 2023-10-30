# rust_realtime_table
Web API project that provides stock price quoting for few stocks.
the following features are supported:


Via API clients can:
 - Submit price changes
 - Get the price of a stock symbol
 - Get a snapshot of prices

Via websockets a client can:
 - Get a snapshot of prices
 - Listen for stock price change event



### Main UI look
![image](https://github.com/jcalahor/rust_realtime_table/assets/7434088/336abd66-a64b-47fb-bc1d-e6ef6190872b)





### Get Table Snapshot
![image](https://github.com/jcalahor/rust_realtime_table/assets/7434088/130b359b-7db4-4392-b019-286ab4bfa1bb)






### Submit a stock price change
![image](https://github.com/jcalahor/rust_realtime_table/assets/7434088/3e184572-4a0e-4dd1-a8f1-669aa898921e)





### Check stock price changed

![image](https://github.com/jcalahor/rust_realtime_table/assets/7434088/a624fb6d-8057-4eb1-85a1-239bbd923451)


###TODOS:

 - Detailed Documentation/Readme
 - Accept price changes via kafka and broadcast to the client sockets
 - Revisit Static Lazy Load vs AppState for certain structs
