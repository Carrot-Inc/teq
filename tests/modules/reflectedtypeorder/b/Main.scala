package rtoc

import rtom.Lists

@main def run(): Unit =
  Lists.of[rto.Ground].foreach(println)
  Lists.of[rto.Tree[Int, Int]].foreach(println)
  Lists.of[rto.Shrub].foreach(println)
  Lists.of[rto.Glade.type].foreach(println)
  Lists.of[rto.Noon].foreach(println)
  Lists.of[rto.Night].foreach(println)
  Lists.of[rto.Board].foreach(println)
  Lists.of[rto.Bin[Int]].foreach(println)
