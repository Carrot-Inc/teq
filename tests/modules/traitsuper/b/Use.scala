package tsb

import tsa.*

class Both extends Greeter, Polite, Loud

object Use:
  def main(args: Array[String]): Unit =
    println(new Both().greet("ann"))
