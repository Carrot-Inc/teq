// jars: scala-library scalajs-dom
// scalajs-dom 2.8.1's facades from its jar in the shapes an application's pages use them:
// `document.getElementById`, an `HTMLInputElement`'s `value` read and written, a `click`
// listener taking a `dom.MouseEvent`, `window.setTimeout`, `window.location`, `localStorage`,
// `querySelectorAll` over a `NodeList`, `classList`, `HTMLElement.style`. Node has no `document`,
// so the program only reaches these when one is present: under node it is compiled and linked
// and prints the guard's line. The expectation is written by hand.
package dombrowser

import org.scalajs.dom
import org.scalajs.dom.{HTMLInputElement, HTMLElement, MouseEvent}
import scala.scalajs.js

def page(): Unit =
  val input = dom.document.getElementById("name").asInstanceOf[HTMLInputElement]
  input.value = input.value.trim
  val button = dom.document.querySelector("button").asInstanceOf[HTMLElement]
  button.addEventListener("click", (e: MouseEvent) => dom.console.log(e.clientX, input.value))
  button.classList.add("ready")
  button.style.display = "none"
  val items = dom.document.querySelectorAll("li")
  for i <- 0 until items.length do items(i).textContent = s"item $i"
  dom.window.setTimeout(() => dom.window.localStorage.setItem("seen", "1"), 10)
  println(dom.window.location.pathname)

@main def main(): Unit =
  if js.typeOf(js.Dynamic.global.document) == "undefined" then println("no document")
  else page()
