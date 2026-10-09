package ijb

import ija.Fail

@main def run(): Unit =
  try Fail.fail("no")
  catch case _: Exception => println("caught")
