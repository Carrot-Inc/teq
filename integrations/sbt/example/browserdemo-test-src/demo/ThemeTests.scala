package demo

import utest.*

import demo.model.{Theme, Themes}

object ThemeTests extends TestSuite:
  val tests = Tests {
    test("the light theme turns dark") {
      assert(Themes.next(Theme.Light) == Theme.Dark(1))
    }
    test("a dark theme deepens") {
      assert(Themes.next(Theme.Dark(4)) == Theme.Dark(5))
    }
  }
