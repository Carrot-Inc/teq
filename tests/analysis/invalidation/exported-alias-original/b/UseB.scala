package eob

import eoa.*

object UseB:
  val car = new Car
  def keep(f: car.Fuel): car.Fuel = f
  def keep2(f: CarObj.Fuel): CarObj.Fuel = f
