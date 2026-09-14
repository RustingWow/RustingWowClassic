UPDATE gossip_text_broadcasts SET
    probability_0 = probability_0 * 100,
    probability_1 = probability_1 * 100,
    probability_2 = probability_2 * 100,
    probability_3 = probability_3 * 100,
    probability_4 = probability_4 * 100,
    probability_5 = probability_5 * 100,
    probability_6 = probability_6 * 100,
    probability_7 = probability_7 * 100
WHERE GREATEST(
    probability_0, probability_1, probability_2, probability_3,
    probability_4, probability_5, probability_6, probability_7
) <= 1;
