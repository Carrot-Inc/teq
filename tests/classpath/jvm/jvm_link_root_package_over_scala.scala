// jars: scala-library
// std: lean scala-library
// A top-level package hides the member of `scala` of its name, which only the root import
// brings: scala-library's annotation class `main` under `--std=scala-library`.
package main:
  case class AppConfig(name: String)

package executor:
  import main.AppConfig

  @main def run(): Unit =
    println(AppConfig("service"))
