# artifact/ -- guideStone data and expected outputs

Data files are NOT committed to git. Fetch from primary sources:

```bash
# Zenodo (full GPS artifact)
wget https://zenodo.org/records/17653393/files/GPS_v5.zip

# LINCS L1000 Level 5 (primary gene expression data)
# See: https://clue.io/data/CMap2020#702

# ChEMBL (bioactivity)
# See: https://www.ebi.ac.uk/chembl/

# NF Data Portal (NF1 transcriptomics)
# See: https://nf.synapse.org/
```

Expected outputs for each module are in `validation/expected/`.
These ARE committed -- they define what reproduction must match.
