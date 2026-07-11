import argparse
import json
import sys
from typing import NamedTuple


class Entry(NamedTuple):
    bank_id: int
    instrument: int
    kind: str
    wave_system: int
    wave: int


def main():
    parser = argparse.ArgumentParser(
        description="Decodes the output of the \"banker\" program, listing the programs in each"
            " sound bank and the sounds used by it.",
    )
    parser.add_argument(dest="input_json", nargs='?', default=None)
    args = parser.parse_args()

    if args.input_json is None:
        input_json = sys.stdin
    else:
        input_json = open(args.input_json, "r", encoding="utf-8")

    with input_json:
        data = json.load(input_json)

    entries = []
    bank_id = data["bank_id"]
    for section in data["sections"]:
        if section["type"] != "List":
            continue
        for i, list_item in enumerate(section["list_items"]):
            if list_item["type"] == "Invalid":
                continue
            if list_item["type"] == "Percussion":
                for percussion_map in list_item["percussion_maps"]:
                    if percussion_map is None:
                        continue
                    for velocity_region in percussion_map["velocity_regions"]:
                        wave_system = velocity_region["wave_system_id"]
                        wave_id = velocity_region["wave_id"]
                        entries.append(Entry(bank_id, i, "P", wave_system, wave_id))
            elif list_item["type"] == "Instrument":
                for key_region in list_item["key_regions"]:
                    for velocity_region in key_region["velocity_regions"]:
                        wave_system = velocity_region["wave_system_id"]
                        wave_id = velocity_region["wave_id"]
                        entries.append(Entry(bank_id, i, "I", wave_system, wave_id))
            else:
                raise ValueError(f"unknown list item type {list_item['type']}")

    for entry in entries:
        print(f"{entry.bank_id:3} {entry.instrument:3} {entry.kind} {entry.wave_system:3} {entry.wave:4}")


if __name__ == "__main__":
    main()
