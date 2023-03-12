#!/bin/env python3

from procmem import ProcessInfo, ProcmemSimple

for p in ProcessInfo.list_all():
	print(str(p))

pid = int(input("pid: "))
app = ProcmemSimple(pid)
print("Process:", app.process_info())

all_pages = app.pages()
pages = []
for page in all_pages:
	if page.permissions.read and page.permissions.write and not page.permissions.shared and page.offset == 0:
		pages.append(page)
		print(f"  {page}")

def print_matches(app, matches, value_type = "i32"):
	for m in matches:
		value = app.read(m, value_type)
		print(f"0x{m:X}: {value}")

def scan(app, value, prev_matches = None, value_type = "i32"):
	matches = app.scan_exact(pages, value, value_type = value_type)
	if prev_matches is not None:
		matches = matches & prev_matches

	matches_len = len(matches)
	if matches_len == 0:
		print("No matches")
		matches = None
	elif matches_len == 1:
		print("1 match:")
		print_matches(app, matches)
	elif matches_len < 5:
		print(f"{matches_len} matches:")
		print_matches(app, matches)
	else:
		print(f"{matches_len} matches")
	
	return matches

matches = None
# matches = scan(app, 231, matches)
# app.write(offset, 500, "i32")