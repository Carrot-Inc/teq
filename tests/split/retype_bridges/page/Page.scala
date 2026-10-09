package page

import lib.*
import cats.syntax.all.*

object Page:
  def show(p: Person): String =
    val city = Model.city.getOption(p).getOrElse("nowhere")
    val street = Model.street.getOption(p).getOrElse("no street")
    "Greeting: " + p.name + " of " + city + ", " + street + " " + p.name.some.filter(_.nonEmpty).getOrElse("?")

  def main(args: Array[String]): Unit =
    val p = Person("Ann", Some(Address("Main", Some("Oslo"))))
    println(show(Model.rename(p, "Bob")))
    println(show(Model.city.replace("Bergen")(p)))
