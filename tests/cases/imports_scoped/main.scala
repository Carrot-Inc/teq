package app
package main

import app.model.LoanModels.*
import ActionType.*
import Limits.{maxItems as limit}
import util.Text
import util.Numbers.*
import _root_.app.route.{*, given}
import app.model.Rules

def render[A](a: A)(using r: Render[A]): String = r.render(a)

@main def run(): Unit =
  val loan = Loan(7, Cancel)
  println(loan)
  println(limit)
  println(Text.shout("relative"))
  println(one)
  println(Rules.flip(Rules.default))
  println(Rules.sample)
  println(Rules.capped(99))
  println(base)
  println(endpoint("loans"))
  println(actionPath(Renew))
  println(render(RouteError.NotFound("/x")))
  println(render(RouteError.Forbidden))
  println(_root_.app.util.Text.shout("rooted"))
  val e: _root_.app.route.RouteError = RouteError.Forbidden
  println(e)
