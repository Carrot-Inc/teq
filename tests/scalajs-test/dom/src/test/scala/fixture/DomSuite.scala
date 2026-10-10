package fixture

import org.scalajs.dom
import org.scalatest.funsuite.AnyFunSuite
import scala.scalajs.js

class DomSuite extends AnyFunSuite:
  test("the program runs in the window's realm") {
    assert(js.Dynamic.global.globalThis.asInstanceOf[js.Any] eq dom.window.asInstanceOf[js.Any])
    assert(js.typeOf(js.Dynamic.global.process) == "undefined")
  }
  test("an element is appended to the document") {
    val p = dom.document.createElement("p")
    p.textContent = "hello"
    dom.document.body.appendChild(p)
    assert(dom.document.querySelectorAll("p").length == 1)
  }
  test("the page is http://localhost/ and keeps storage") {
    assert(dom.window.location.href == "http://localhost/")
    dom.window.localStorage.setItem("key", "value")
    assert(dom.window.localStorage.getItem("key") == "value")
  }
