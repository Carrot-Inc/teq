package p

trait Base:
  def value: Int

trait Factory:
  def make(): Base
