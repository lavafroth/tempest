#!/usr/bin/env bash
mkdir -p models/april models/bart
wget "https://april.sapples.net/aprilv0_en-us.april" -c -O models/april/model.april
wget "https://huggingface.co/facebook/bart-large-mnli/resolve/main/rust_model.ot" -c -O models/bart/model.ot
wget "https://huggingface.co/facebook/bart-large-mnli/resolve/main/config.json" -c -O models/bart/config.json
wget "https://huggingface.co/roberta-large/resolve/main/vocab.json" -c -O models/bart/vocab.json
wget "https://huggingface.co/roberta-large/resolve/main/merges.txt" -c -O models/bart/merges.txt
