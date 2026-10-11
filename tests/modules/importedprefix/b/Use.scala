package use

import importedprefix.*

object Use:
  def main(args: Array[String]): Unit =
    println(new O(10).make(new O(20)))
    println(new O(100).curried(new O(20)))
