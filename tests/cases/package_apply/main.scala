package app

@main def main(): Unit =
  println(views.activity(1))
  println(views.activity("names"))
  println(views.activity.apply(2))
  println(views.nested)
