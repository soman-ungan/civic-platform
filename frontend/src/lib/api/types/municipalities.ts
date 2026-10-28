// SPDX-License-Identifier: AGPL-3.0-only

// 自治体の API の型（backend/src/interface/api/municipalities.rs）

export type Municipality = {
  id: string;
  slug: string;
  name: string;
  prefecture_name: string;
};

export type District = {
  id: string;
  name: string;
  name_kana: string;
};

export type MunicipalityDetail = Municipality & {
  districts: District[];
};
