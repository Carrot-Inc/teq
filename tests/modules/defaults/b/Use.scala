package db

import da.*

object Use:
  def main(args: Array[String]): Unit =
    val c = new Client(port = 8080)
    println(c.url())
    println(c.url(secure = true))
    println(c.query("k")(1))
    println(Client.default().url("/x"))
