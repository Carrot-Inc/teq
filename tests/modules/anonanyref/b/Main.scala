package aarb

import aar.Api

@main def run(): Unit =
  println(Api.make())
  println(Api.obj())
  println(Api.counted())
  println(Api.named().name + " " + Api.both().name)
