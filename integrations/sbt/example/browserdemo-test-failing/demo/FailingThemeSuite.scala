package demo

import demo.model.{Theme, Themes}

class FailingThemeSuite extends munit.FunSuite:
  test("the dark theme stays at its level") {
    assertEquals(Themes.next(Theme.Dark(1)), Theme.Dark(1))
  }
