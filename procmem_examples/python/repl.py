#!/bin/env python3

from procmem import ProcessInfo, ProcmemSimple

class Matches:
	def __init__(self, max_history = 2):
		self.max_history = max_history
		self.history = []
		self.current = None
	
	def add(self, new_matches):
		if self.current is not None:
			# defensive >=
			if len(self.history) >= self.max_history:
				self.history.pop(0)
			self.history.append(self.current)
			new_matches = new_matches & self.current
		
		self.current = new_matches
	
	def __iter__(self):
		if self.current is None:
			return iter([])
		
		return iter(self.current)

	def print(self, app, value_type = "i32"):
		if self.current is None:
			print("No matches")
		else:
			l = len(self.current)
			if l <= 5:
				self.print_all(app, value_type)
			else:
				print(f"{l} matches")
	
	def print_all(self, app, value_type = "i32"):
		for (off, size) in self.current:
			# TODO: size
			value = app.read(off, value_type)
			print(f"0x{off:X}(+{size}): {value}")
	
	def undo(self):
		if len(self.history) > 0:
			self.current = self.history.pop()
		else:
			self.current = None

APP = None
PAGES = None
MATCHES = Matches()
VALUE_TYPE = "i32"

def initialize():
	global APP
	
	for p in ProcessInfo.list_all():
		print(str(p))

	pid = int(input("pid: "))
	APP = ProcmemSimple(pid)
	print("Process:", APP.process_info())

def reinitialize():
	global APP

	pid = APP.process_info().pid
	APP = None
	APP = ProcmemSimple(pid)

def get_pages():
	global PAGES

	pages = []
	for page in APP.pages():
		if page.permissions.read and page.permissions.write and not page.permissions.shared and page.offset == 0:
			pages.append(page)
			print(f"  {page}")
	
	PAGES = pages

def scan(value):
	if MATCHES.current is None:
		new_matches = APP.scan_exact_pages(PAGES, value, value_type = VALUE_TYPE)
	else:
		new_matches = APP.scan_exact_addresses(list(MATCHES.current), value, value_type = VALUE_TYPE)
	MATCHES.add(new_matches)
	
	MATCHES.print(APP, VALUE_TYPE)


def write(value, offset = None):
	if offset is None:
		for (off, size) in MATCHES:
			APP.write(off, value, VALUE_TYPE)
	else:
		APP.write(offset, value, VALUE_TYPE)

if __name__ == "__main__":
	initialize()
	get_pages()
	# scan(123)
	# scan(456)
	# write(50000)
