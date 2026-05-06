#!/usr/bin/env python3
"""Write drug_fields.json with all 60 drugs' data."""
import json
import os

TOOLS_DIR = os.path.dirname(os.path.abspath(__file__))

DRUG_DATA = {}
