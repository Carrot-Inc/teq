package client

import apimodel.route.*
import apimodel.route.sub.Deep

@main def run(): Unit =
  println(Users.list)
  println(Users.ids.map(_.value))
  println(Deep.p)
  println(apimodel.route.base)
  println("x".slash)
