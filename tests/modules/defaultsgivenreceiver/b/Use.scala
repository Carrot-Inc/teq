package dgivuse

import dgiv.*

@main def main(): Unit =
  locally { import Givens.given; Givens.use }
