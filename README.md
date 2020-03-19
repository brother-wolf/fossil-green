# Fossil Green

This is a rust application that reports fossil fuel usage of your AWS account.
It is based on the limited information that AWS provide around energy sources for data-centres.

A large credit has to go to [green-cost-explorer](https://github.com/thegreenwebfoundation/green-cost-explorer/blob/master/AWS-Regions.png) by [thegreenwebfoundation](https://github.com/thegreenwebfoundation) which I couldn't get to work but the idea, code, and navigation of the AWS Cost Explorer, is very sound.

This application takes 3 parameters:

* aws-profile (String)
* start-date (String of format: YYYY-MM-DD)
* end-date (String of format: YYYY-MM-DD)

