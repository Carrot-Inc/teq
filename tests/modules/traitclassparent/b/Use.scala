package tcpb

class Failed extends tcpa.Failure
class Hushed extends tcpa.Quiet

@main def run(): Unit =
  println(new Failed().isInstanceOf[Exception])
  try throw new Failed catch case e: Failed => println("caught Failed")
  try throw new Hushed catch case e: Hushed => println("caught Hushed " + e.getStackTrace.length)
