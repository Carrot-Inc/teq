package dinluse

import dinl.Api

@main def main(): Unit =
  Api.simple()
  Api.order()
  Api.nested()
  println(Api.f()(Api.mark("D")))
