package demo

import utest.*

import demo.model.{Theme, Themes}

object FailingThemeTests extends TestSuite:
  val tests = Tests {
    test("the light theme goes back to light") {
      assert(Themes.next(Theme.Light) == Theme.Light)
    }
  }
