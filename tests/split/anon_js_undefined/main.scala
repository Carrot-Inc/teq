package app

import scala.scalajs.js
import api.Options

@main def main(): Unit =
  val o = new Options {
    val req = "hello"
  }
  println(js.Object.keys(o).mkString(","))
  println(js.JSON.stringify(o))
