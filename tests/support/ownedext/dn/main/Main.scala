package main

import shared.CommonUtils.*
import shared.{Lib, Platform}

case class Service(portShift: Option[Int])
case class Raw(service: Service)

object Mine extends Platform:
  def base = 7
  object A extends Auto

object Launcher:
  def main(args: Array[String]): Unit =
    val rawConf = Raw(Service(Some(3)))
    val conf = if rawConf.service.portShift.optionHolds(3) then "offset" else "plain"
    println(conf)
    println(Set(1, 2).setHolds(2))
    println(5.in(1, 5))
    println(Lib.Impl.value)
    println(Lib.Impl.viaOuter)
    println(Mine.A.value)
    println(Mine.A.viaOuter)
    println(shared.Relay.authorized)
