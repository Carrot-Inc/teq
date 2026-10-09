object UseMacroWorkers5:
  def use(x: Any): Boolean = MacroWorkers.isString(x)
